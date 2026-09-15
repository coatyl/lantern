//! Network boundary for Lantern: dead-link checker.
//!
//! # Design
//!
//! The checker is **opt-in**: every build of `lantern-net` exposes the result
//! types so the rest of the workspace can refer to them, but the actual HTTP
//! machinery (`reqwest`, `tokio`, TLS) is gated behind the `checker` feature
//! so that disabling the dead-link checker setting in the UI also disables the
//! associated dependency tree.
//!
//! # API
//!
//! - [`LinkStatus`]: outcome of probing a single URL.
//! - [`LinkResult`]: `(url, status, elapsed_ms)` triple.
//! - [`classify_status`]: pure function mapping a numeric HTTP status code
//!   into a [`LinkStatus`].  Always available.
//! - [`should_check`]: predicate filtering which URLs are worth probing
//!   (skips `chrome:`, `about:`, `javascript:`, …).  Always available.
//! - [`check_links`]: async batch checker.  Only available with the
//!   `checker` feature.
//!
//! # Privacy note
//!
//! The PRD requires the dead-link checker to be off by default (NFR-PRIV-2).
//! Callers must surface the opt-in toggle (`AppSettings::dead_link_checker_opt_in`)
//! before invoking [`check_links`].

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Result types: always available
// ---------------------------------------------------------------------------

/// Outcome of probing a single URL.
///
/// Serialised with an internal `kind` tag so the JSON shape is stable for IPC
/// consumers (every variant becomes `{ "kind": "<name>", … }` with the variant
/// payload spread alongside the tag).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LinkStatus {
    /// 2xx: page exists.
    Ok { code: u16 },
    /// 3xx: observed during redirect chasing (final hop is reported as `Ok`
    /// or another error class instead).  Only emitted when redirect-following
    /// is disabled.
    Redirect { code: u16 },
    /// 4xx: link is broken from the client side (404, 410, …).
    ClientError { code: u16 },
    /// 5xx: server-side error; the link may still be reachable later.
    ServerError { code: u16 },
    /// Request did not finish within the configured timeout.
    Timeout,
    /// DNS / TCP / TLS failure or other transport-level problem.
    NetworkError { detail: String },
    /// URL did not parse, or used an unsupported scheme.
    Skipped { reason: SkipReason },
}

/// Why a URL was not probed at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// `url::Url::parse` rejected the input.
    InvalidUrl,
    /// Scheme is not `http` / `https` (`chrome:`, `about:`, `javascript:`, …).
    UnsupportedScheme,
    /// Empty / whitespace-only string.
    Empty,
}

impl LinkStatus {
    /// True when the status indicates the link is reachable (2xx).
    pub fn is_ok(&self) -> bool {
        matches!(self, LinkStatus::Ok { .. })
    }

    /// True when the status indicates a permanent client failure (4xx).
    /// Used by the UI to flag candidates for deletion.
    pub fn is_dead(&self) -> bool {
        matches!(self, LinkStatus::ClientError { .. })
    }
}

/// One row in the checker's report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkResult {
    pub url: String,
    pub status: LinkStatus,
    /// Time-to-status, milliseconds.  `0` for skipped URLs.
    pub elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// Status classification: pure helper
// ---------------------------------------------------------------------------

/// Map an HTTP status code into a [`LinkStatus`].
///
/// Used by [`check_links`] internally and exposed for tests and for code that
/// has obtained a status code through other means (e.g. a custom transport in
/// integration tests).
pub fn classify_status(code: u16, follow_redirects: bool) -> LinkStatus {
    match code {
        100..=199 => LinkStatus::NetworkError {
            detail: format!("unexpected informational status {code}"),
        },
        200..=299 => LinkStatus::Ok { code },
        300..=399 => {
            if follow_redirects {
                // The follower should have replaced this with the final hop;
                // if we still see a 3xx it usually means the redirect chain
                // exceeded `max_redirects`.
                LinkStatus::NetworkError {
                    detail: format!("redirect chain not resolved (status {code})"),
                }
            } else {
                LinkStatus::Redirect { code }
            }
        }
        400..=499 => LinkStatus::ClientError { code },
        500..=599 => LinkStatus::ServerError { code },
        _ => LinkStatus::NetworkError {
            detail: format!("unexpected status {code}"),
        },
    }
}

// ---------------------------------------------------------------------------
// URL filtering: pure helper
// ---------------------------------------------------------------------------

/// Decide whether `raw` is worth sending across the network.
///
/// Returns either the parsed URL (caller can hand it to reqwest unchanged) or
/// the [`SkipReason`] explaining why it was rejected.  The function never
/// performs I/O.
pub fn should_check(raw: &str) -> Result<url::Url, SkipReason> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SkipReason::Empty);
    }
    let parsed = url::Url::parse(trimmed).map_err(|_| SkipReason::InvalidUrl)?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        _ => Err(SkipReason::UnsupportedScheme),
    }
}

// ---------------------------------------------------------------------------
// Configuration: always available
// ---------------------------------------------------------------------------

/// Knobs for the dead-link checker.
///
/// Construct with [`CheckOptions::default`] and tweak as needed.  Fields are
/// public for ergonomics; the defaults are tuned for "kind" probing: moderate
/// concurrency, a generous timeout, and a polite User-Agent.
#[derive(Debug, Clone)]
pub struct CheckOptions {
    /// Per-request timeout (connection + headers).
    pub timeout: std::time::Duration,
    /// How many requests to keep in-flight at once.
    pub concurrency: usize,
    /// User-Agent header value.  Many sites short-circuit unrecognised UAs.
    pub user_agent: String,
    /// If true, the checker silently follows up to `max_redirects` hops and
    /// reports the final status; if false, the first 3xx is surfaced as
    /// [`LinkStatus::Redirect`].
    pub follow_redirects: bool,
    /// Cap on redirect hops; ignored when `follow_redirects = false`.
    pub max_redirects: usize,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(10),
            concurrency: 8,
            user_agent: format!(
                "Lantern/{} (+https://github.com/lantern-app)",
                env!("CARGO_PKG_VERSION")
            ),
            follow_redirects: true,
            max_redirects: 5,
        }
    }
}

// ---------------------------------------------------------------------------
// Async checker: feature-gated
// ---------------------------------------------------------------------------

#[cfg(feature = "checker")]
mod checker {
    use super::*;
    use futures::stream::{FuturesUnordered, StreamExt};
    use std::time::Instant;

    /// Probe a batch of URLs and return one [`LinkResult`] per input.
    ///
    /// Order of the output array matches the input.  Concurrency is capped by
    /// `opts.concurrency`; URLs that fail [`should_check`] are returned with
    /// [`LinkStatus::Skipped`] without ever touching the network.
    ///
    /// Cancellation: dropping the returned future cancels in-flight requests.
    pub async fn check_links(urls: &[String], opts: &CheckOptions) -> Vec<LinkResult> {
        let client = match build_client(opts) {
            Ok(c) => c,
            Err(e) => {
                // If the client itself can't be built, fail every entry the
                // same way so the UI shows a coherent error per row.
                return urls
                    .iter()
                    .map(|u| LinkResult {
                        url: u.clone(),
                        status: LinkStatus::NetworkError {
                            detail: format!("client init failed: {e}"),
                        },
                        elapsed_ms: 0,
                    })
                    .collect();
            }
        };

        // Pre-classify: anything we'd skip never enters the request stream.
        // We still need a stable index to preserve input order in the output.
        let prepared: Vec<(usize, String, Result<url::Url, SkipReason>)> = urls
            .iter()
            .enumerate()
            .map(|(i, u)| (i, u.clone(), should_check(u)))
            .collect();

        // Reserve the output array up front so we can write results by index.
        let mut out: Vec<Option<LinkResult>> = (0..urls.len()).map(|_| None).collect();

        // Bounded concurrency via FuturesUnordered + a sliding window.
        let mut iter = prepared.into_iter();
        let mut inflight = FuturesUnordered::new();

        // Prime the pipeline.
        let cap = opts.concurrency.max(1);
        for _ in 0..cap {
            if let Some(work) = iter.next() {
                inflight.push(probe_one(client.clone(), opts.clone(), work));
            } else {
                break;
            }
        }

        while let Some(done) = inflight.next().await {
            let (idx, result) = done;
            out[idx] = Some(result);
            if let Some(work) = iter.next() {
                inflight.push(probe_one(client.clone(), opts.clone(), work));
            }
        }

        out.into_iter()
            .map(|r| r.expect("every slot must be filled"))
            .collect()
    }

    fn build_client(opts: &CheckOptions) -> Result<reqwest::Client, reqwest::Error> {
        let redirect_policy = if opts.follow_redirects {
            reqwest::redirect::Policy::limited(opts.max_redirects)
        } else {
            reqwest::redirect::Policy::none()
        };
        reqwest::Client::builder()
            .timeout(opts.timeout)
            .user_agent(&opts.user_agent)
            .redirect(redirect_policy)
            .build()
    }

    async fn probe_one(
        client: reqwest::Client,
        opts: CheckOptions,
        work: (usize, String, Result<url::Url, SkipReason>),
    ) -> (usize, LinkResult) {
        let (idx, raw, parsed) = work;

        let parsed = match parsed {
            Ok(p) => p,
            Err(reason) => {
                return (
                    idx,
                    LinkResult {
                        url: raw,
                        status: LinkStatus::Skipped { reason },
                        elapsed_ms: 0,
                    },
                )
            }
        };

        let start = Instant::now();

        // Prefer HEAD; most servers answer it cheaply.  Some servers return
        // 405 Method Not Allowed for HEAD; we retry once with GET in that case.
        let head = client.head(parsed.clone()).send().await;
        let status = match head {
            Ok(resp) if resp.status().as_u16() == 405 => match client.get(parsed).send().await {
                Ok(resp) => classify_status(resp.status().as_u16(), opts.follow_redirects),
                Err(e) => map_request_error(e),
            },
            Ok(resp) => classify_status(resp.status().as_u16(), opts.follow_redirects),
            Err(e) => map_request_error(e),
        };

        let elapsed_ms = start.elapsed().as_millis() as u64;
        (
            idx,
            LinkResult {
                url: raw,
                status,
                elapsed_ms,
            },
        )
    }

    fn map_request_error(e: reqwest::Error) -> LinkStatus {
        if e.is_timeout() {
            LinkStatus::Timeout
        } else {
            LinkStatus::NetworkError {
                detail: e.to_string(),
            }
        }
    }
}

#[cfg(feature = "checker")]
pub use checker::check_links;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_2xx_is_ok() {
        assert_eq!(classify_status(200, true), LinkStatus::Ok { code: 200 });
        assert_eq!(classify_status(204, true), LinkStatus::Ok { code: 204 });
        assert_eq!(classify_status(299, true), LinkStatus::Ok { code: 299 });
    }

    #[test]
    fn classify_4xx_is_client_error() {
        assert_eq!(
            classify_status(404, true),
            LinkStatus::ClientError { code: 404 }
        );
        assert_eq!(
            classify_status(410, true),
            LinkStatus::ClientError { code: 410 }
        );
        assert!(classify_status(404, true).is_dead());
        assert!(!classify_status(200, true).is_dead());
    }

    #[test]
    fn classify_5xx_is_server_error() {
        assert_eq!(
            classify_status(500, true),
            LinkStatus::ServerError { code: 500 }
        );
        assert_eq!(
            classify_status(503, true),
            LinkStatus::ServerError { code: 503 }
        );
        assert!(!classify_status(503, true).is_dead());
    }

    #[test]
    fn classify_3xx_with_follow_is_unresolved_chain() {
        match classify_status(301, true) {
            LinkStatus::NetworkError { .. } => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn classify_3xx_without_follow_is_redirect() {
        assert_eq!(
            classify_status(301, false),
            LinkStatus::Redirect { code: 301 }
        );
        assert_eq!(
            classify_status(308, false),
            LinkStatus::Redirect { code: 308 }
        );
    }

    #[test]
    fn classify_unknown_codes_are_network_errors() {
        match classify_status(999, true) {
            LinkStatus::NetworkError { .. } => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn should_check_accepts_http_and_https() {
        assert!(should_check("https://example.com/").is_ok());
        assert!(should_check("http://example.com/").is_ok());
        // Whitespace is trimmed.
        assert!(should_check("  https://example.com/  ").is_ok());
    }

    #[test]
    fn should_check_rejects_unsupported_schemes() {
        assert_eq!(
            should_check("javascript:alert(1)").unwrap_err(),
            SkipReason::UnsupportedScheme
        );
        assert_eq!(
            should_check("chrome://newtab").unwrap_err(),
            SkipReason::UnsupportedScheme
        );
        assert_eq!(
            should_check("about:blank").unwrap_err(),
            SkipReason::UnsupportedScheme
        );
        assert_eq!(
            should_check("data:text/html,hi").unwrap_err(),
            SkipReason::UnsupportedScheme
        );
    }

    #[test]
    fn should_check_rejects_empty() {
        assert_eq!(should_check("").unwrap_err(), SkipReason::Empty);
        assert_eq!(should_check("   ").unwrap_err(), SkipReason::Empty);
    }

    #[test]
    fn should_check_rejects_invalid_url() {
        assert_eq!(
            should_check("not a url").unwrap_err(),
            SkipReason::InvalidUrl
        );
    }

    #[test]
    fn link_status_is_ok_only_for_2xx() {
        assert!(LinkStatus::Ok { code: 200 }.is_ok());
        assert!(!LinkStatus::ClientError { code: 404 }.is_ok());
        assert!(!LinkStatus::Timeout.is_ok());
        assert!(!LinkStatus::Skipped {
            reason: SkipReason::Empty
        }
        .is_ok());
    }

    #[test]
    fn link_result_round_trips_through_serde_json() {
        let r = LinkResult {
            url: "https://example.com/".into(),
            status: LinkStatus::ClientError { code: 404 },
            elapsed_ms: 42,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: LinkResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn default_options_are_polite() {
        let o = CheckOptions::default();
        assert!(o.timeout >= std::time::Duration::from_secs(5));
        assert!(o.concurrency >= 1);
        assert!(o.user_agent.starts_with("Lantern/"));
        assert!(o.follow_redirects);
    }

    #[cfg(feature = "checker")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn check_links_skips_unsupported_schemes_without_io() {
        // Use an unrouteable host and a tight timeout; if the checker actually
        // tried to hit the network for the http URL we'd notice via timing.
        // The skipped entries must come back instantly.
        let urls = vec![
            "javascript:alert(1)".to_string(),
            "about:blank".to_string(),
            "".to_string(),
            "definitely not a url".to_string(),
        ];
        let opts = CheckOptions {
            timeout: std::time::Duration::from_millis(50),
            concurrency: 4,
            ..CheckOptions::default()
        };

        let start = std::time::Instant::now();
        let results = check_links(&urls, &opts).await;
        let elapsed = start.elapsed();

        assert_eq!(results.len(), 4);
        assert!(matches!(
            results[0].status,
            LinkStatus::Skipped {
                reason: SkipReason::UnsupportedScheme
            }
        ));
        assert!(matches!(
            results[1].status,
            LinkStatus::Skipped {
                reason: SkipReason::UnsupportedScheme
            }
        ));
        assert!(matches!(
            results[2].status,
            LinkStatus::Skipped {
                reason: SkipReason::Empty
            }
        ));
        assert!(matches!(
            results[3].status,
            LinkStatus::Skipped {
                reason: SkipReason::InvalidUrl
            }
        ));
        // Should be near-instant (no awaiting on network).
        assert!(elapsed < std::time::Duration::from_secs(1));
    }

    #[cfg(feature = "checker")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn check_links_preserves_input_order() {
        let urls = vec![
            "javascript:1".to_string(),
            "about:blank".to_string(),
            "".to_string(),
        ];
        let results = check_links(&urls, &CheckOptions::default()).await;
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].url, "javascript:1");
        assert_eq!(results[1].url, "about:blank");
        assert_eq!(results[2].url, "");
    }
}
