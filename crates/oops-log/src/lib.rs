//! Logging set up the same way in every tool: [`tracing`] with the configuration done once.
//!
//! ```no_run
//! let _guard = oops_log::Logging::new("orbistoun").init();
//! tracing::info!("started");
//! ```
//!
//! Log with [`tracing`]'s own macros; this crate does not wrap them, so `#[instrument]` and
//! structured fields keep working.
//!
//! # The guard
//!
//! [`Logging::init`] returns a [`Guard`] that must live as long as the program. The file and
//! OTLP writers batch in the background and stop when it drops. `let _guard = ...` keeps it;
//! `let _ = ...` drops it at once.
//!
//! # Levels
//!
//! The first of these that is set wins:
//!
//! 1. `OOPS_LOG`
//! 2. `RUST_LOG`
//! 3. [`Logging::level`], default `info`
//!
//! The variables take the full [`EnvFilter`] syntax: `OOPS_LOG=warn,orbistoun_loader=debug`.
//! When neither is set and the level is `info` or more verbose, `wgpu`, `wgpu_core`,
//! `wgpu_hal` and `naga` are capped at `warn`; they log every frame at `info`.
//!
//! [`EnvFilter`]: tracing_subscriber::EnvFilter
//!
//! # Destinations
//!
//! Stderr always, never stdout, which tools pipe. A rolling file with the `file` feature. OTLP
//! with the `otlp` feature. The default build compiles neither extra destination.

use tracing_subscriber::EnvFilter;
#[cfg(feature = "otlp")]
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub use tracing::Level;

/// The collection's level variable, read before `RUST_LOG`.
pub const LEVEL_ENV: &str = "OOPS_LOG";

/// How a tool wants its logging set up. Built with [`Logging::new`], turned on with
/// [`Logging::init`].
#[derive(Debug, Clone)]
pub struct Logging {
    service: String,
    level: Level,
    ansi: bool,
    build: Option<String>,
    root: Option<std::path::PathBuf>,
    #[cfg(feature = "file")]
    directory: Option<std::path::PathBuf>,
    #[cfg(feature = "otlp")]
    endpoint: Option<String>,
}

impl Logging {
    /// Starts configuring. `service` is the tool's name as a person would say it (`pros`,
    /// `obscene-tool`), and is what an aggregator groups by.
    #[must_use]
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            level: Level::INFO,
            ansi: true,
            build: None,
            root: None,
            #[cfg(feature = "file")]
            directory: None,
            #[cfg(feature = "otlp")]
            endpoint: None,
        }
    }

    /// The build, for the startup line. Pass `oops_build::line!()`.
    #[must_use]
    pub fn build(mut self, build: impl Into<String>) -> Self {
        self.build = Some(build.into());
        self
    }

    /// Where the tool keeps its data, for the startup line. Pass `oops_paths::Paths::data_root`.
    #[must_use]
    pub fn root(mut self, root: impl Into<std::path::PathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    /// The level used when neither environment variable is set.
    #[must_use]
    pub const fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Turns colour off, for output that is captured.
    #[must_use]
    pub const fn without_colour(mut self) -> Self {
        self.ansi = false;
        self
    }

    /// Also writes a daily rolling file in `directory`, typically `Paths::logs_dir()`.
    #[cfg(feature = "file")]
    #[must_use]
    pub fn to_file(mut self, directory: impl Into<std::path::PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// Also exports over OTLP to `endpoint`, e.g. `http://localhost:4317`. The caller must
    /// already be inside a tokio runtime.
    #[cfg(feature = "otlp")]
    #[must_use]
    pub fn to_otlp(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// The filter when no variable is set: `level` everywhere, with the render crates capped
    /// at `warn` when `level` is more verbose than `warn`. At `warn` or quieter the cap would
    /// raise them, so it is not added.
    fn default_filter(level: Level) -> EnvFilter {
        let base = EnvFilter::new(level.to_string());
        if level <= Level::WARN {
            return base;
        }
        ["wgpu=warn", "wgpu_core=warn", "wgpu_hal=warn", "naga=warn"]
            .into_iter()
            .fold(base, |filter, directive| {
                filter.add_directive(
                    directive
                        .parse()
                        .expect("a literal render-crate directive parses"),
                )
            })
    }

    /// Turns logging on and returns the [`Guard`] to hold for the life of the program.
    ///
    /// A second call does nothing and logs that at `debug`. A destination that cannot be set
    /// up (an unwritable directory, an unreachable collector) is reported on stderr and
    /// skipped; the tool still starts.
    #[must_use = "dropping the guard stops the file and OTLP writers"]
    pub fn init(self) -> Guard {
        let filter = EnvFilter::try_from_env(LEVEL_ENV)
            .or_else(|_| EnvFilter::try_from_default_env())
            .unwrap_or_else(|_| Self::default_filter(self.level));

        let stderr = tracing_subscriber::fmt::layer()
            .with_ansi(self.ansi)
            .with_writer(std::io::stderr);

        #[cfg(feature = "file")]
        let mut worker = None;
        #[cfg(feature = "file")]
        let file = self.directory.as_ref().and_then(|dir| {
            if let Err(error) = std::fs::create_dir_all(dir) {
                eprintln!("logging: no file in {}: {error}", dir.display());
                return None;
            }
            let appender = tracing_appender::rolling::daily(dir, format!("{}.log", self.service));
            let (writer, keep) = tracing_appender::non_blocking(appender);
            worker = Some(keep);
            Some(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_writer(writer),
            )
        });
        #[cfg(not(feature = "file"))]
        let file: Option<tracing_subscriber::fmt::Layer<_>> = None;

        #[cfg(feature = "otlp")]
        let mut exporting = false;
        #[cfg(feature = "otlp")]
        let otlp =
            self.endpoint
                .as_ref()
                .and_then(|endpoint| match otlp_layer(&self.service, endpoint) {
                    Ok(layer) => {
                        exporting = true;
                        Some(layer)
                    }
                    Err(error) => {
                        eprintln!("logging: no OTLP export to {endpoint}: {error}");
                        None
                    }
                });

        let registry = tracing_subscriber::registry()
            .with(filter)
            .with(stderr)
            .with(file);
        #[cfg(feature = "otlp")]
        let registry = registry.with(otlp);

        if registry.try_init().is_err() {
            tracing::debug!(
                service = %self.service,
                "logging was already initialised; this call did nothing"
            );
            return Guard::inert(self.service);
        }
        // The build and the data root, which every bug report needs. At `debug`, so an
        // ordinary run is silent, and after the subscriber exists so it is recorded.
        tracing::debug!(
            service = %self.service,
            build = self.build.as_deref().unwrap_or("unstamped"),
            root = ?self.root,
            "starting"
        );
        Guard {
            service: self.service,
            #[cfg(feature = "file")]
            file: worker,
            #[cfg(feature = "otlp")]
            otlp: exporting,
        }
    }
}

/// The OTLP layer for `service`, exporting to `endpoint`.
#[cfg(feature = "otlp")]
fn otlp_layer<S>(
    service: &str,
    endpoint: &str,
) -> Result<impl Layer<S>, opentelemetry::trace::TraceError>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_otlp::WithExportConfig as _;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(opentelemetry_sdk::Resource::new([
            opentelemetry::KeyValue::new("service.name", service.to_owned()),
        ]))
        .build();
    let tracer = provider.tracer(service.to_owned());
    opentelemetry::global::set_tracer_provider(provider);
    Ok(tracing_opentelemetry::layer().with_tracer(tracer))
}

/// Keeps the background writers alive; hold it for the life of the program.
#[must_use = "dropping this stops the file and OTLP writers"]
pub struct Guard {
    service: String,
    /// Held only to be dropped, which flushes the file appender.
    #[cfg(feature = "file")]
    #[allow(dead_code)]
    file: Option<tracing_appender::non_blocking::WorkerGuard>,
    #[cfg(feature = "otlp")]
    otlp: bool,
}

impl Guard {
    /// A guard that owns nothing, returned when logging was already set up.
    fn inert(service: String) -> Self {
        Self {
            service,
            #[cfg(feature = "file")]
            file: None,
            #[cfg(feature = "otlp")]
            otlp: false,
        }
    }

    /// The service name this was set up under.
    #[must_use]
    pub fn service(&self) -> &str {
        &self.service
    }
}

impl std::fmt::Debug for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Guard")
            .field("service", &self.service)
            .finish_non_exhaustive()
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        // Flushes the OTLP batch; the file appender's guard flushes itself.
        #[cfg(feature = "otlp")]
        if self.otlp {
            opentelemetry::global::shutdown_tracer_provider();
        }
    }
}

/// Stderr at the level the environment asks for: `Logging::new(service).init()`.
#[must_use = "dropping the guard stops the file and OTLP writers"]
pub fn init(service: impl Into<String>) -> Guard {
    Logging::new(service).init()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A second initialisation in one process is ignored, not a panic.
    #[test]
    fn a_second_initialisation_is_ignored_rather_than_fatal() {
        let _first = Logging::new("test").init();
        let _second = Logging::new("test").init();
    }

    /// The builder stores what it is given.
    #[test]
    fn the_builder_keeps_what_it_is_given() {
        let built = Logging::new("orbistoun")
            .level(Level::DEBUG)
            .without_colour();
        assert_eq!(built.service, "orbistoun");
        assert_eq!(built.level, Level::DEBUG);
        assert!(!built.ansi);
    }

    /// The variable name is referenced by CI and scripts across the collection.
    #[test]
    fn the_level_variable_is_the_one_the_collection_agrees_on() {
        assert_eq!(LEVEL_ENV, "OOPS_LOG");
    }

    /// The render crates are capped at verbose levels and left alone at quiet ones.
    #[test]
    fn the_default_holds_wgpu_down_only_when_that_is_quieter() {
        for verbose in [Level::INFO, Level::DEBUG, Level::TRACE] {
            let shown = Logging::default_filter(verbose).to_string();
            assert!(
                shown.contains("wgpu_core=warn"),
                "at {verbose} the render crates should be capped: {shown}"
            );
        }
        for quiet in [Level::WARN, Level::ERROR] {
            let shown = Logging::default_filter(quiet).to_string();
            assert!(
                !shown.contains("wgpu"),
                "at {quiet} nothing should be raised to warn: {shown}"
            );
        }
    }
}
