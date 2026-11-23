//! Retry-After, rate-limit reset and HTTP-date parsing.

use opi_ai::retry::{parse_http_date_delay, parse_retry_after};

#[test]
fn parse_retry_after_seconds_value() {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("retry-after", "30".parse().unwrap());
    let ms = parse_retry_after(&headers);
    assert_eq!(ms, Some(30_000));
}

#[test]
fn parse_retry_after_no_header_returns_none() {
    let headers = reqwest::header::HeaderMap::new();
    assert_eq!(parse_retry_after(&headers), None);
}

#[test]
fn parse_retry_after_x_ratelimit_reset() {
    let mut headers = reqwest::header::HeaderMap::new();
    let future_ts = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 60) as f64;
    headers.insert("x-ratelimit-reset", future_ts.to_string().parse().unwrap());
    let ms = parse_retry_after(&headers);
    let ms = ms.expect("should parse x-ratelimit-reset");
    assert!(
        (55_000..=65_000).contains(&ms),
        "expected ~60000ms, got {ms}"
    );
}

#[test]
fn parse_retry_after_prefers_retry_after_over_ratelimit() {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("retry-after", "10".parse().unwrap());
    headers.insert("x-ratelimit-reset", "9999999999".parse().unwrap());
    assert_eq!(parse_retry_after(&headers), Some(10_000));
}

#[test]
fn parse_retry_after_invalid_value_returns_none() {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("retry-after", "not-a-number".parse().unwrap());
    assert_eq!(parse_retry_after(&headers), None);
}

#[test]
fn parse_http_date_valid_future_date() {
    // Construct a date ~120 seconds from now
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 120;
    let date_str = unix_to_http_date(future);
    let delay = parse_http_date_delay(&date_str).expect("should parse future HTTP-date");
    assert!(
        (110_000..=130_000).contains(&delay),
        "expected ~120000ms, got {delay}"
    );
}

#[test]
fn parse_http_date_past_returns_none() {
    // 01 Jan 2020 is in the past
    let date_str = "Wed, 01 Jan 2020 00:00:00 GMT";
    assert_eq!(parse_http_date_delay(date_str), None);
}

#[test]
fn parse_http_date_invalid_format_returns_none() {
    assert_eq!(parse_http_date_delay("not a date"), None);
    assert_eq!(parse_http_date_delay("Fri May 23 12:00:00 2026"), None);
    assert_eq!(parse_http_date_delay("Fri, 23 May 2026"), None);
}

#[test]
fn parse_http_date_wrong_timezone_returns_none() {
    assert_eq!(parse_http_date_delay("Fri, 23 May 2026 12:00:00 EST"), None);
}

#[test]
fn parse_retry_after_with_http_date_header() {
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 30;
    let date_str = unix_to_http_date(future);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("retry-after", date_str.parse().unwrap());
    let ms = parse_retry_after(&headers);
    let ms = ms.expect("should parse HTTP-date Retry-After header");
    assert!(
        (25_000..=35_000).contains(&ms),
        "expected ~30000ms, got {ms}"
    );
}

// Helper: convert Unix timestamp to IMF-fixdate string.
fn unix_to_http_date(ts: u64) -> String {
    let days = ts / 86400;
    let time_secs = ts % 86400;
    let hour = time_secs / 3600;
    let minute = (time_secs % 3600) / 60;
    let second = time_secs % 60;

    // Civil date from days since epoch (Howard Hinnant algorithm)
    let z = days as i64 + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    let weekday = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][(days % 7) as usize];
    let month_name = [
        "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][m as usize];

    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        weekday, d, month_name, y, hour, minute, second
    )
}
