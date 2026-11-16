//! HTTP helpers shared by the providers: turning "slow down" responses into
//! a typed error the scheduler can act on.

use std::{
    fmt,
    time::{Duration, SystemTime},
};

use anyhow::{Error, Result};
use reqwest::{header::RETRY_AFTER, Response, StatusCode};

use super::schedule::{fmt_duration, MAX_RETRY_AFTER};

/// The service told us to back off (HTTP 429, or 503 with a `Retry-After`).
#[derive(Debug)]
pub struct RateLimited {
    /// How long it asked us to wait, if it said.
    pub retry_after: Option<Duration>,
}

impl fmt::Display for RateLimited {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.retry_after {
            Some(wait) => write!(f, "rate limited by the service; it asked us to wait {}", fmt_duration(wait)),
            None => write!(f, "rate limited by the service"),
        }
    }
}

impl std::error::Error for RateLimited {}

/// Is this a "slow down" response (HTTP 429, a spent quota, or 503 with `Retry-After`)?
fn rate_limited(response: &Response) -> Option<RateLimited> {
    let status = response.status();
    let retry_after = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| parse_retry_after(v, SystemTime::now()));

    // Some services report an exhausted quota as 403 with this header
    let quota_spent = status == StatusCode::FORBIDDEN
        && response
        .headers()
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.trim() == "0");
    let limited = status == StatusCode::TOO_MANY_REQUESTS
        || quota_spent
        || (status == StatusCode::SERVICE_UNAVAILABLE && retry_after.is_some());
    limited.then_some(RateLimited { retry_after })
}

/// Like `Response::error_for_status`, but a "slow down" response becomes a
/// `RateLimited` error instead of an anonymous one, so its `Retry-After`
/// isn't lost.
pub fn check_status(response: Response) -> Result<Response> {
    if let Some(limit) = rate_limited(&response) {
        return Err(Error::new(limit));
    }
    Ok(response.error_for_status()?)
}

/// Like `check_status`, but a failure also carries the start of the response
/// body, which usually says what was wrong (a rejected token, a missing scope).
pub async fn check_status_with_body(response: Response) -> Result<Response> {
    if let Some(limit) = rate_limited(&response) {
        return Err(Error::new(limit));
    }
    let status = response.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return Ok(response);
    }
    let url = response.url().clone();
    let body = response.text().await.unwrap_or_default();
    let snippet: String = body.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(200).collect();
    Err(anyhow::anyhow!("HTTP {status} for {url}: {snippet}"))
}

/// The rate-limit error inside `error`, however many layers of context wrap it.
pub fn rate_limit(error: &Error) -> Option<&RateLimited> {
    error.chain().find_map(|cause| cause.downcast_ref::<RateLimited>())
}

/// `Retry-After` is either a number of seconds or an HTTP date. Absurdly long
/// waits are capped, and a date in the past means "no extra wait".
pub fn parse_retry_after(value: &str, now: SystemTime) -> Option<Duration> {
    let value = value.trim();
    let wait = match value.parse::<u64>() {
        Ok(secs) => Duration::from_secs(secs),
        Err(_) => httpdate::parse_http_date(value)
            .ok()?
            .duration_since(now)
            .unwrap_or(Duration::ZERO),
    };
    Some(wait.min(MAX_RETRY_AFTER))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    fn response(status: u16, retry_after: Option<&str>) -> Response {
        let mut builder = http::Response::builder().status(status);
        if let Some(value) = retry_after {
            builder = builder.header("retry-after", value);
        }
        Response::from(builder.body("").unwrap())
    }

    #[test]
    fn a_403_with_no_quota_left_counts_as_a_rate_limit() {
        let spent = http::Response::builder().status(403).header("x-ratelimit-remaining", "0").body("").unwrap();
        assert!(rate_limit(&check_status(Response::from(spent)).unwrap_err()).is_some());

        let left = http::Response::builder().status(403).header("x-ratelimit-remaining", "12").body("").unwrap();
        assert!(rate_limit(&check_status(Response::from(left)).unwrap_err()).is_none());
        assert!(rate_limit(&check_status(response(403, None)).unwrap_err()).is_none());
    }

    #[tokio::test]
    async fn a_failure_with_a_body_says_what_the_service_said() {
        let rejected = http::Response::builder()
            .status(403)
            .body(r#"{"error":"insufficient_scope",  "error_description":"The access token lacks the browse scope"}"#)
            .unwrap();
        let error = check_status_with_body(Response::from(rejected)).await.unwrap_err();
        let text = error.to_string();
        assert!(text.contains("403") && text.contains("insufficient_scope"), "{text}");
        assert!(rate_limit(&error).is_none());

        // success passes through untouched, and a 429 is still a rate limit
        assert!(check_status_with_body(response(200, None)).await.is_ok());
        let error = check_status_with_body(response(429, Some("5"))).await.unwrap_err();
        assert_eq!(rate_limit(&error).unwrap().retry_after, Some(Duration::from_secs(5)));
    }

    #[tokio::test]
    async fn a_long_body_is_shortened() {
        let long = "x".repeat(5000);
        let failed = http::Response::builder().status(500).body(long).unwrap();
        let error = check_status_with_body(Response::from(failed)).await.unwrap_err();
        assert!(error.to_string().len() < 400);
    }

    #[test]
    fn retry_after_as_seconds() {
        let now = SystemTime::now();
        assert_eq!(parse_retry_after("120", now), Some(Duration::from_secs(120)));
        assert_eq!(parse_retry_after(" 5 ", now), Some(Duration::from_secs(5)));
        assert_eq!(parse_retry_after("0", now), Some(Duration::ZERO));
    }

    #[test]
    fn retry_after_as_an_http_date() {
        // 21 Oct 2015 07:28:00 GMT is 1445412480 seconds after the epoch
        let date = "Wed, 21 Oct 2015 07:28:00 GMT";
        let thirty_before = UNIX_EPOCH + Duration::from_secs(1_445_412_480 - 30);
        assert_eq!(parse_retry_after(date, thirty_before), Some(Duration::from_secs(30)));
        let long_after = UNIX_EPOCH + Duration::from_secs(1_445_412_480 + 1000);
        assert_eq!(parse_retry_after(date, long_after), Some(Duration::ZERO));
    }

    #[test]
    fn nonsense_and_huge_values() {
        let now = SystemTime::now();
        assert_eq!(parse_retry_after("soon", now), None);
        assert_eq!(parse_retry_after("-5", now), None);
        assert_eq!(parse_retry_after("", now), None);
        assert_eq!(parse_retry_after("99999999999", now), Some(MAX_RETRY_AFTER));
    }

    #[test]
    fn a_429_becomes_a_rate_limit_error_with_its_wait() {
        let error = check_status(response(429, Some("7"))).unwrap_err();
        assert_eq!(rate_limit(&error).unwrap().retry_after, Some(Duration::from_secs(7)));

        let error = check_status(response(429, None)).unwrap_err();
        assert_eq!(rate_limit(&error).unwrap().retry_after, None);
    }

    #[test]
    fn a_503_only_counts_when_it_says_how_long() {
        let error = check_status(response(503, Some("30"))).unwrap_err();
        assert!(rate_limit(&error).is_some());

        let error = check_status(response(503, None)).unwrap_err();
        assert!(rate_limit(&error).is_none());
    }

    #[test]
    fn other_statuses_behave_as_before() {
        assert!(check_status(response(200, None)).is_ok());
        let error = check_status(response(404, None)).unwrap_err();
        assert!(rate_limit(&error).is_none());
    }

    #[test]
    fn the_error_is_found_through_added_context() {
        use anyhow::Context;
        let error = check_status(response(429, Some("3")))
            .context("Wallhaven: could not list wallpapers")
            .unwrap_err();
        assert!(rate_limit(&error).is_some());
        assert!(format!("{error:#}").contains("asked us to wait 3s"));
    }
}
