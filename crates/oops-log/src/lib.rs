//! Turning logging on, the same way in every tool.
//!
//! Nothing here is novel and that is deliberate. It is [`tracing`] underneath - the facade the
//! Rust ecosystem already agrees on - and this crate is the small amount of glue that stops four
//! projects each inventing their own answer to "where do the lines go and how loud are they".
//!
//! ```no_run
//! let _guard = oops_log::Logging::new("orbistoun").init();
//! tracing::info!("started");
//! ```
//!
//! Use [`tracing`]'s macros directly - `error!`, `warn!`, `info!`, `debug!`, `trace!`. This crate
//! does not wrap them and should not: a wrapper would break `#[instrument]`, structured fields
//! and every editor that knows what `tracing` is.
//!
//! # The guard is load-bearing
//!
//! [`Logging::init`] returns a [`Guard`] that must be **held for the life of the program**. The
//! file writer batches on a background thread and the OTLP exporter batches over the network;
//! dropping the guard early stops both, and the symptom is a log file that is empty or missing
//! its last few seconds - the part somebody was reading it for. `let _ = ...` drops it
//! immediately. `let _guard = ...` does not.
//!
//! # Levels
//!
//! Resolution order, first match wins:
//!
//! 1. `OOPS_LOG` - the same variable for every tool in the collection.
//! 2. `RUST_LOG` - because everyone's fingers already know it.
//! 3. Whatever [`Logging::level`] was given, defaulting to `info`.
//!
//! Both variables take the full [`EnvFilter`] syntax, so a directive can be per-module:
//! `OOPS_LOG=warn,orbistoun_loader=debug`.
//!
//! [`EnvFilter`]: tracing_subscriber::EnvFilter
//!
//! # Where the lines go
//!
//! Stderr always, because a tool that logs to stdout corrupts whatever is being piped out of it.
//! A rolling file with the `file` feature. OTLP with the `otlp` feature - that is the wire
//! format an LGTM stack ingests, so "send it to Grafana" needs no code here beyond an endpoint.
//!
//! Each destination is a cargo feature rather than a runtime dependency, so a tool that wants a
//! level and nothing else compiles neither.

use tracing_subscriber::EnvFilter;
#[cfg(feature = "otlp")]
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub use tracing::Level;

/// The variable this collection reads, in preference to `RUST_LOG`.
pub const LEVEL_ENV: &str = "OOPS_LOG";

/// How a tool wants its logging set up.
///
/// Built with [`Logging::new`] and turned on with [`Logging::init`].
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
    /// Start configuring, naming the service.
    ///
    /// The name is what a log aggregator groups by, so it should be the tool a person would say
    /// they were running - `orbistoun`, `pros`, `obscene-tool` - not the crate that happens to
    /// call this.
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

    /// Which build this is, for the startup line.
    ///
    /// Pass `oops_build::line!()`. Kept as a plain string rather than a dependency on
    /// `oops-build`, so this crate stays usable by anything and the two are not welded together.
    #[must_use]
    pub fn build(mut self, build: impl Into<String>) -> Self {
        self.build = Some(build.into());
        self
    }

    /// Where the tool is keeping its data, for the startup line.
    ///
    /// Pass `oops_paths::Paths::data_root`. Same reasoning as [`Logging::build`].
    #[must_use]
    pub fn root(mut self, root: impl Into<std::path::PathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    /// The level to use when neither environment variable is set.
    #[must_use]
    pub const fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Turn colour off.
    ///
    /// Worth doing when the output is known to be captured - a CI log full of escape sequences
    /// is harder to read than a plain one, not easier.
    #[must_use]
    pub const fn without_colour(mut self) -> Self {
        self.ansi = false;
        self
    }

    /// Also write to a rolling daily file in `directory`.
    ///
    /// Takes a directory rather than a file so the rotation has somewhere to put yesterday's.
    /// Pair it with `oops-paths` (`logs_dir()`) rather than choosing a location here - where a
    /// tool keeps its data is that tool's decision and it is already made once.
    #[cfg(feature = "file")]
    #[must_use]
    pub fn to_file(mut self, directory: impl Into<std::path::PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// Also export over OTLP to `endpoint`, e.g. `http://localhost:4317`.
    ///
    /// The consumer must already be inside a tokio runtime; the exporter batches on it.
    #[cfg(feature = "otlp")]
    #[must_use]
    pub fn to_otlp(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Turn it on.
    ///
    /// # Returns
    ///
    /// A [`Guard`] that must be held for the life of the program - see the crate note. Dropping
    /// it early truncates the file and the OTLP batch.
    ///
    /// # Calling this twice
    ///
    /// The second call does nothing and says so at `debug`, rather than panicking. A test
    /// harness that initialises per-test, or a library helpfully setting up logging for a
    /// binary that already did, is a mistake worth neither a crash nor silence.
    #[must_use = "dropping the guard stops the file and OTLP writers"]
    pub fn init(self) -> Guard {
        let filter = EnvFilter::try_from_env(LEVEL_ENV)
            .or_else(|_| EnvFilter::try_from_default_env())
            .unwrap_or_else(|_| EnvFilter::new(self.level.to_string()));

        // Stderr, never stdout: a tool that logs to stdout corrupts whatever is being piped
        // out of it, and every one of these tools has an output somebody redirects.
        let stderr = tracing_subscriber::fmt::layer()
            .with_ansi(self.ansi)
            .with_writer(std::io::stderr);

        #[cfg(feature = "file")]
        let mut worker = None;
        #[cfg(feature = "file")]
        let file = self.directory.as_ref().and_then(|dir| {
            // A logging setup that fails the program is worse than one that logs less. An
            // unwritable directory is reported to stderr - which is already working - and the
            // run continues.
            if let Err(error) = std::fs::create_dir_all(dir) {
                eprintln!("logging: no file in {}: {error}", dir.display());
                return None;
            }
            let appender = tracing_appender::rolling::daily(dir, format!("{}.log", self.service));
            let (writer, keep) = tracing_appender::non_blocking(appender);
            worker = Some(keep);
            // Never coloured: escape sequences in a file are noise to every reader of it.
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
        let otlp = self.endpoint.as_ref().and_then(|endpoint| {
            match otlp_layer(&self.service, endpoint) {
                Ok(layer) => {
                    exporting = true;
                    Some(layer)
                }
                Err(error) => {
                    // Same rule as the file: a collector that is not there is a fact about the
                    // environment, not a reason for the tool to fail to start.
                    eprintln!("logging: no OTLP export to {endpoint}: {error}");
                    None
                }
            }
        });

        let registry = tracing_subscriber::registry()
            .with(filter)
            .with(stderr)
            .with(file);
        #[cfg(feature = "otlp")]
        let registry = registry.with(otlp);

        if registry.try_init().is_err() {
            // Naming the loser matters: when two components both set logging up, "already
            // initialised" without a name leaves you guessing which one won.
            tracing::debug!(
                service = %self.service,
                "logging was already initialised; this call did nothing"
            );
            return Guard::inert(self.service);
        }
        // **Which build, and where it writes.** The two facts every bug report needs and nobody
        // remembers to ask for. Emitted here rather than by each tool because seven binaries
        // hand-writing the same four lines is seven chances to word it differently, log it at
        // the wrong level, or forget it - and it has to come after the subscriber exists, which
        // is a detail every one of them would have to get right separately.
        //
        // `debug`, so an ordinary run stays silent.
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

/// Builds the OTLP layer, or explains why it could not.
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

/// Keeps the background writers alive.
///
/// Hold this for the life of the program. See the crate note on why `let _ =` is the wrong way
/// to receive it.
#[must_use = "dropping this stops the file and OTLP writers"]
pub struct Guard {
    service: String,
    /// Never read, and that is the point: this exists to be *dropped*, at which moment the
    /// appender flushes what it has buffered. Reading it would do nothing. The compiler cannot
    /// tell that apart from a field somebody forgot about, so the exemption is stated here
    /// rather than switched off for the crate.
    #[cfg(feature = "file")]
    #[allow(dead_code)]
    file: Option<tracing_appender::non_blocking::WorkerGuard>,
    #[cfg(feature = "otlp")]
    otlp: bool,
}

impl Guard {
    /// A guard that owns nothing, for the call that found logging already set up.
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
        // The exporter batches, so whatever is queued when the program ends is lost unless it
        // is told to finish. The file appender's own guard handles itself.
        #[cfg(feature = "otlp")]
        if self.otlp {
            opentelemetry::global::shutdown_tracer_provider();
        }
    }
}

/// The common case: stderr, at whatever level the environment asks for.
///
/// Held separately from the builder because most tools want exactly this and should not have to
/// read a builder's documentation to get it.
#[must_use = "dropping the guard stops the file and OTLP writers"]
pub fn init(service: impl Into<String>) -> Guard {
    Logging::new(service).init()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_initialisation_is_ignored_rather_than_fatal() {
        // Both calls in one test on purpose: tests share a process, and a panic-on-second-init
        // would make this crate unusable from any test that logs.
        let _first = Logging::new("test").init();
        let _second = Logging::new("test").init();
    }

    #[test]
    fn the_builder_keeps_what_it_is_given() {
        let built = Logging::new("orbistoun")
            .level(Level::DEBUG)
            .without_colour();
        assert_eq!(built.service, "orbistoun");
        assert_eq!(built.level, Level::DEBUG);
        assert!(!built.ansi);
    }

    #[test]
    fn the_level_variable_is_the_one_the_collection_agrees_on() {
        // Pinned because it is written into CI, run scripts and documentation across four
        // repositories; renaming it silently turns every one of those into a no-op.
        assert_eq!(LEVEL_ENV, "OOPS_LOG");
    }
}
