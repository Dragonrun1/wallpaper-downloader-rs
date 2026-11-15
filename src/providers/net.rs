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

/// Like `Response::error_for_status`, but a "slow down" response becomes a
/// `RateLimited` error instead of an anonymous one, so its `Retry-After`
/// isn't lost.
pub fn check_status(response: Response) -> Result<Response> {
    let status = response.status();
    let retry_after = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| parse_retry_after(v, SystemTime::now()));

    let limited = status == StatusCode::TOO_MANY_REQUESTS
        || (status == StatusCode::SERVICE_UNAVAILABLE && retry_after.is_some());
    if limited {
        return Err(Error::new(RateLimited { retry_after }));
    }
    Ok(response.error_for_status()?)
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
