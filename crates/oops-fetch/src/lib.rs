//! Pure fallback and verification policy for multi-source asset fetching.
//!
//! Shared across tools that need to resolve payloads from multiple candidate sources
//! (e.g. Orbistoun and Prosperous).
//!
//! Neither HTTP clients nor manifest domain models belong here (D001, D008). Callers
//! supply the fetch mechanism and the digest check as closures; this crate encodes:
//!
//! 1. **Local-first ordering**: Local filesystem candidates are tried before remote mirrors.
//! 2. **Refuse before fetch**: Remote sources are refused without attempting any download
//!    if no checkable digest is available.
//! 3. **Verification inside policy**: When a digest is present, fetched bytes are verified
//!    before being accepted.
//! 4. **Complete failure reporting**: Every candidate source and its failure reason is
//!    recorded and reported if all sources fail.

/// Which category an origin belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OriginKind {
    /// A path on this machine.
    Local,
    /// Something to fetch over the network (`http://` or `https://`).
    Remote,
}

impl OriginKind {
    /// Classifies an origin string.
    ///
    /// Only `http://` and `https://` are remote. Everything else — including Windows drive
    /// paths like `C:\...` whose colon is a drive separator rather than a URL scheme — is local.
    #[must_use]
    pub fn classify(origin: &str) -> Self {
        if origin.starts_with("http://") || origin.starts_with("https://") {
            Self::Remote
        } else {
            Self::Local
        }
    }

    /// Whether this origin is local.
    #[must_use]
    pub fn is_local(self) -> bool {
        matches!(self, Self::Local)
    }

    /// Whether this origin is remote.
    #[must_use]
    pub fn is_remote(self) -> bool {
        matches!(self, Self::Remote)
    }
}

/// A classified origin string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Origin {
    /// A local filesystem path.
    Local(String),
    /// A network resource (`http://` or `https://`).
    Remote(String),
}

impl Origin {
    /// Classifies an origin string into an [`Origin`].
    ///
    /// Only `http://` and `https://` are remote. Everything else — including Windows drive
    /// paths like `C:\...` whose colon is a drive separator rather than a URL scheme — is local.
    #[must_use]
    pub fn classify(origin: &str) -> Self {
        if origin.starts_with("http://") || origin.starts_with("https://") {
            Self::Remote(origin.to_owned())
        } else {
            Self::Local(origin.to_owned())
        }
    }

    /// The origin kind.
    #[must_use]
    pub fn kind(&self) -> OriginKind {
        match self {
            Self::Local(_) => OriginKind::Local,
            Self::Remote(_) => OriginKind::Remote,
        }
    }

    /// The underlying origin string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Local(s) | Self::Remote(s) => s.as_str(),
        }
    }

    /// Whether this origin is local.
    #[must_use]
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }

    /// Whether this origin is remote.
    #[must_use]
    pub fn is_remote(&self) -> bool {
        matches!(self, Self::Remote(_))
    }
}

/// What happened when an origin list was walked and a candidate succeeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt<T> {
    /// What the origin that answered and verified produced.
    pub value: T,
    /// Which origin was used.
    pub used: String,
    /// Whether the used origin was local.
    pub is_local: bool,
    /// The origins tried before it, and why each failed.
    pub failed: Vec<(String, String)>,
}

/// Why an asset walk could not provide a verified payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// No origins were provided to walk.
    NoSources,
    /// Only remote sources were provided, but no checkable digest was available to verify them.
    Unverifiable,
    /// Every candidate origin was attempted and failed.
    AllFailed(Vec<(String, String)>),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSources => write!(f, "no sources provided"),
            Self::Unverifiable => {
                write!(
                    f,
                    "refusing to download remote sources: no checkable digest"
                )
            }
            Self::AllFailed(failed) => {
                write!(f, "all sources failed: ")?;
                for (i, (origin, why)) in failed.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{origin}: {why}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for FetchError {}

/// Reorders origins so that local origins precede remote origins, preserving order within each group.
#[must_use]
pub fn order_local_first<'a, S: AsRef<str> + ?Sized>(origins: &'a [&'a S]) -> Vec<&'a str> {
    let mut local = Vec::new();
    let mut remote = Vec::new();
    for origin in origins {
        let s = origin.as_ref();
        if OriginKind::classify(s).is_local() {
            local.push(s);
        } else {
            remote.push(s);
        }
    }
    local.extend(remote);
    local
}

/// Walks origins in local-first order and returns the first that fetches and verifies.
///
/// # Invariants
///
/// 1. **Local-first ordering**: Local paths are tried before remote URLs, preserving order
///    within each group.
/// 2. **Refuse before fetch**: Remote sources are never downloaded if no checkable digest is
///    available (`has_digest == false`). If only remote sources exist, [`FetchError::Unverifiable`]
///    is returned without calling `fetch`.
/// 3. **Verification inside policy**: When `has_digest` is true, every fetched payload is verified
///    via `verify` before acceptance. If verification fails, the payload is rejected and the next
///    origin is tried.
/// 4. **Complete failure reporting**: If no source succeeds, all attempted origins and reasons are
///    returned in [`FetchError::AllFailed`].
///
/// # Errors
///
/// - [`FetchError::NoSources`] when `origins` is empty.
/// - [`FetchError::Unverifiable`] when only remote origins are given but `has_digest` is false.
/// - [`FetchError::AllFailed`] when every candidate origin failed to fetch or verify.
pub fn walk<T, E: std::fmt::Display>(
    origins: &[impl AsRef<str>],
    has_digest: bool,
    mut fetch: impl FnMut(&str) -> Result<T, E>,
    mut verify: impl FnMut(&T) -> Result<(), E>,
) -> Result<Attempt<T>, FetchError> {
    if origins.is_empty() {
        return Err(FetchError::NoSources);
    }

    let origin_strs: Vec<&str> = origins.iter().map(AsRef::as_ref).collect();
    let ordered = order_local_first(&origin_strs);
    let has_local = ordered.iter().any(|s| OriginKind::classify(s).is_local());

    if !has_local && !has_digest {
        return Err(FetchError::Unverifiable);
    }

    let mut failed = Vec::new();
    for origin in ordered {
        let is_local = OriginKind::classify(origin).is_local();

        if !is_local && !has_digest {
            failed.push((
                origin.to_string(),
                "refused: remote source without checkable digest".to_string(),
            ));
            continue;
        }

        match fetch(origin) {
            Ok(value) => {
                if has_digest {
                    match verify(&value) {
                        Ok(()) => {
                            return Ok(Attempt {
                                value,
                                used: origin.to_string(),
                                is_local,
                                failed,
                            });
                        }
                        Err(why) => {
                            failed.push((
                                origin.to_string(),
                                format!("digest verification failed: {why}"),
                            ));
                        }
                    }
                } else {
                    return Ok(Attempt {
                        value,
                        used: origin.to_string(),
                        is_local: true,
                        failed,
                    });
                }
            }
            Err(why) => {
                failed.push((origin.to_string(), why.to_string()));
            }
        }
    }

    Err(FetchError::AllFailed(failed))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An origin is classified as local or remote from the string.
    #[test]
    fn an_origin_knows_whether_it_is_remote_or_local() {
        assert_eq!(
            Origin::classify("https://github.com/project-oops/releases/download/v1/a.elf"),
            Origin::Remote("https://github.com/project-oops/releases/download/v1/a.elf".to_owned())
        );
        assert_eq!(
            Origin::classify("http://example.invalid/a.bin"),
            Origin::Remote("http://example.invalid/a.bin".to_owned())
        );
        assert_eq!(
            Origin::classify("../obscene/build/prospero"),
            Origin::Local("../obscene/build/prospero".to_owned())
        );
        assert_eq!(
            Origin::classify(r"D:\builds\probe"),
            Origin::Local(r"D:\builds\probe".to_owned())
        );
        assert_eq!(
            Origin::classify(r"C:\Windows\System32"),
            Origin::Local(r"C:\Windows\System32".to_owned())
        );
        assert_eq!(
            OriginKind::classify("https://example.com"),
            OriginKind::Remote
        );
        assert_eq!(OriginKind::classify("file.txt"), OriginKind::Local);
    }

    /// Sources are reordered local first, keeping relative ordering within groups.
    #[test]
    fn sources_are_ordered_local_first() {
        let sources = [
            "https://mirror1.invalid/a.elf",
            "/local/build/a.elf",
            "https://mirror2.invalid/a.elf",
            "relative/path/a.elf",
        ];
        let ordered = order_local_first(&sources);
        assert_eq!(
            ordered,
            vec![
                "/local/build/a.elf",
                "relative/path/a.elf",
                "https://mirror1.invalid/a.elf",
                "https://mirror2.invalid/a.elf",
            ]
        );
    }

    /// Remote-only entries with no checkable digest are refused before fetching anything.
    #[test]
    fn a_remote_only_source_with_no_digest_is_refused_before_fetch() {
        let sources = [
            "https://mirror1.invalid/a.elf",
            "https://mirror2.invalid/a.elf",
        ];
        let mut fetch_calls = 0;
        let result: Result<Attempt<Vec<u8>>, FetchError> = walk(
            &sources,
            false,
            |_| {
                fetch_calls += 1;
                Ok::<Vec<u8>, &str>(b"payload".to_vec())
            },
            |_| Ok::<(), &str>(()),
        );
        assert_eq!(result, Err(FetchError::Unverifiable));
        assert_eq!(fetch_calls, 0);
    }

    /// A local source can be fetched without a digest pin.
    #[test]
    fn a_local_source_without_digest_is_allowed() {
        let sources = ["/local/build/a.elf", "https://mirror.invalid/a.elf"];
        let mut fetch_calls = 0;
        let result = walk(
            &sources,
            false,
            |origin| {
                fetch_calls += 1;
                assert_eq!(origin, "/local/build/a.elf");
                Ok::<Vec<u8>, &str>(b"local payload".to_vec())
            },
            |_| Ok::<(), &str>(()),
        );
        let attempt = result.expect("local fetch should succeed");
        assert_eq!(attempt.used, "/local/build/a.elf");
        assert!(attempt.is_local);
        assert_eq!(attempt.value, b"local payload");
        assert_eq!(fetch_calls, 1);
    }

    /// If a local source fails and remaining sources are remote with no digest, remote is refused.
    #[test]
    fn failed_local_with_unverifiable_remote_reports_refusal() {
        let sources = ["/local/missing.elf", "https://mirror.invalid/a.elf"];
        let mut fetch_calls = 0;
        let result: Result<Attempt<Vec<u8>>, FetchError> = walk(
            &sources,
            false,
            |origin| {
                fetch_calls += 1;
                Err(format!("file not found: {origin}"))
            },
            |_| Ok(()),
        );
        assert_eq!(fetch_calls, 1);
        let Err(FetchError::AllFailed(failed)) = result else {
            panic!("expected AllFailed");
        };
        assert_eq!(failed.len(), 2);
        assert_eq!(failed[0].0, "/local/missing.elf");
        assert!(failed[0].1.contains("file not found"));
        assert_eq!(failed[1].0, "https://mirror.invalid/a.elf");
        assert!(failed[1].1.contains("refused: remote source"));
    }

    /// The first source that fetches and verifies is kept; earlier failures are recorded.
    #[test]
    fn the_first_source_that_fetches_and_verifies_wins() {
        let sources = [
            "https://mirror1.invalid/a.elf",
            "/local/build/a.elf",
            "https://mirror2.invalid/a.elf",
        ];
        let result = walk(
            &sources,
            true,
            |origin| match origin {
                "/local/build/a.elf" => Err("missing"),
                "https://mirror1.invalid/a.elf" => Ok(b"bad digest".to_vec()),
                "https://mirror2.invalid/a.elf" => Ok(b"good digest".to_vec()),
                _ => unreachable!(),
            },
            |bytes| {
                if bytes == b"good digest" {
                    Ok(())
                } else {
                    Err("hash mismatch")
                }
            },
        );
        let attempt = result.expect("mirror 2 should succeed");
        assert_eq!(attempt.used, "https://mirror2.invalid/a.elf");
        assert!(!attempt.is_local);
        assert_eq!(attempt.value, b"good digest");
        assert_eq!(attempt.failed.len(), 2);
        assert_eq!(
            attempt.failed[0],
            ("/local/build/a.elf".to_string(), "missing".to_string())
        );
        assert_eq!(
            attempt.failed[1],
            (
                "https://mirror1.invalid/a.elf".to_string(),
                "digest verification failed: hash mismatch".to_string()
            )
        );
    }

    /// Corrupt bytes fail verification and fall back to the next source.
    #[test]
    fn corrupt_bytes_fail_verification_and_fall_back() {
        let sources = ["/local/corrupt.elf", "/local/good.elf"];
        let result = walk(
            &sources,
            true,
            |origin| {
                if origin == "/local/corrupt.elf" {
                    Ok(b"corrupted".to_vec())
                } else {
                    Ok(b"valid".to_vec())
                }
            },
            |bytes| {
                if bytes == b"valid" {
                    Ok(())
                } else {
                    Err("corrupt")
                }
            },
        );
        let attempt = result.expect("fallback should succeed");
        assert_eq!(attempt.used, "/local/good.elf");
        assert_eq!(attempt.failed.len(), 1);
        assert!(attempt.failed[0].1.contains("verification failed"));
    }

    /// Empty source list reports `NoSources`.
    #[test]
    fn empty_sources_reports_no_sources() {
        let sources: [&str; 0] = [];
        let result: Result<Attempt<()>, FetchError> = walk(
            &sources,
            true,
            |_| Ok::<(), &str>(()),
            |()| Ok::<(), &str>(()),
        );
        assert_eq!(result, Err(FetchError::NoSources));
    }
}
